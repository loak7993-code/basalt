/*
 * Basalt Engine
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

package dev.leonxlnx.basalt.engine

import dev.leonxlnx.basalt.engine.model.GcPolicy
import dev.leonxlnx.basalt.engine.model.LibraryEntry
import dev.leonxlnx.basalt.engine.model.LibraryRule
import dev.leonxlnx.basalt.engine.model.MemoryPolicy
import dev.leonxlnx.basalt.engine.model.Platform
import dev.leonxlnx.basalt.engine.model.VersionManifest
import java.io.File
import kotlin.io.path.createTempDirectory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Uses the real Mojang 26.3 manifest as the fixture: the tests assert against
 * the actual data the engine will face in production, not synthetic samples.
 */
class JvmArgsBuilderTest {

    private val manifest: VersionManifest by lazy {
        val stream = checkNotNull(
            JvmArgsBuilderTest::class.java.getResourceAsStream("/26.3.json")
        ) { "26.3.json fixture missing" }
        VersionManifest.parse(stream.readBytes().decodeToString())
    }

    private fun layout(root: File) = InstallLayout(
        clientJar = File(root, "versions/26.3/26.3.jar"),
        librariesDir = File(root, "libraries"),
        nativesDir = File(root, "natives"),
        gameDir = File(root, ".minecraft"),
        assetsDir = File(root, "assets"),
        assetIndexId = manifest.assetIndex?.id ?: "34",
        runtimeHome = File(root, "runtime")
    )

    private val auth = JvmArgsBuilder.AuthContext(
        playerName = "basalt_test",
        uuid = "11112222333344445555666677778888"
    )

    private fun builder(
        root: File,
        memory: MemoryPolicy? = MemoryPolicy(heapMb = 1637),
        gc: GcPolicy? = GcPolicy.TunedG1,
        extraJvm: List<String> = emptyList(),
        quickPlay: JvmArgsBuilder.QuickPlay? = null,
        activeProcessors: Int? = 8
    ) = JvmArgsBuilder(
        manifest = manifest,
        platform = Platform.LINUX_X86_64,
        layout = layout(root),
        memory = memory,
        gc = gc,
        auth = auth,
        versionName = "26.3",
        extraJvmArgs = extraJvm,
        activeProcessors = activeProcessors
    )

    private fun tmpRoot(): File =
        createTempDirectory("basalt-engine-test").toFile()

    @Test
    fun `real 26_3 manifest parses with expected shape`() {
        assertEquals("26.3", manifest.id)
        assertEquals(25, manifest.javaVersion?.majorVersion)
        assertEquals(114, manifest.libraries.size)
        assertEquals("net.minecraft.client.main.Main", manifest.mainClass)
    }

    @Test
    fun `classpath on linux x86_64 contains exactly allowed libraries plus client`() {
        val root = tmpRoot()
        val plan = builder(root).build()

        val allowed = manifest.libraries.filter {
            LibraryFilter.isAllowed(it, Platform.LINUX_X86_64)
        }
        // client jar first, then one entry per allowed library that carries an artifact
        val withArtifacts = allowed.filter { it.downloads?.artifact?.path != null }
        assertEquals(1 + withArtifacts.size, plan.classpath.size)
        assertTrue(plan.classpath.first().endsWith("26.3.jar"))
        val distinctPaths = plan.classpath.drop(1).map { File(it).path }
        assertEquals(distinctPaths.size, distinctPaths.toSet().size)
    }

    @Test
    fun `linux natives for lwjgl are on the classpath, osx ones are not`() {
        val root = tmpRoot()
        val plan = builder(root).build()
        val lwjglNativeEntries = plan.classpath.filter {
            it.contains("lwjgl") && it.contains("natives-linux")
        }
        assertTrue(lwjglNativeEntries.isNotEmpty(), "expected linux lwjgl natives on classpath")
        assertFalse(plan.classpath.any { it.contains("natives-macos") })
        assertFalse(plan.classpath.any { it.contains("natives-windows") })
    }

    @Test
    fun `memory policy replaces manifest heap flags`() {
        val root = tmpRoot()
        val jvm = builder(root, memory = MemoryPolicy(heapMb = 1637)).buildJvmArgs()

        // manifest default-user-jvm carries -Xms2G/-Xmx4G; our policy wins
        assertFalse(jvm.contains("-Xms2G"))
        assertFalse(jvm.contains("-Xmx4G"))
        assertTrue(jvm.contains("-Xms1637M"))
        assertTrue(jvm.contains("-Xmx1637M"))
    }

    @Test
    fun `memory policy clamps into the configured window`() {
        val low = MemoryPolicy(heapMb = 100, minMb = 512, maxMb = 4096)
        assertEquals(512, low.effectiveHeapMb)
        val high = MemoryPolicy(heapMb = 90000, minMb = 512, maxMb = 8192)
        assertEquals(8192, high.effectiveHeapMb)
    }

    @Test
    fun `tuned G1 policy strips manifest collector selection`() {
        val root = tmpRoot()
        val jvm = builder(root, gc = GcPolicy.TunedG1).buildJvmArgs()

        // manifest default-user-jvm prescribes UseZGC on linux; tuned G1 wins
        assertFalse(jvm.contains("-XX:+UseZGC"))
        assertTrue(jvm.contains("-XX:+UseG1GC"))
        assertTrue(jvm.contains("-XX:G1NewSizePercent=20"))
        assertTrue(jvm.contains("-XX:G1ReservePercent=20"))
        assertTrue(jvm.contains("-XX:MaxGCPauseMillis=40"))
        assertTrue(jvm.contains("-XX:G1HeapRegionSize=16M"))
        assertTrue(jvm.contains("-XX:+DisableExplicitGC"))
    }

    @Test
    fun `manifest object-layout flags survive GC override`() {
        val root = tmpRoot()
        val jvm = builder(root, gc = GcPolicy.TunedG1).buildJvmArgs()
        assertTrue(jvm.contains("-XX:+UseCompactObjectHeaders"))
    }

    @Test
    fun `AlwaysPreTouch is dropped for phone-class heaps and kept for large ones`() {
        val root = tmpRoot()
        val small = builder(root, memory = MemoryPolicy(heapMb = 1637)).buildJvmArgs()
        assertFalse(small.contains("-XX:+AlwaysPreTouch"))

        val large = builder(root, memory = MemoryPolicy(heapMb = 6144, minMb = 512, maxMb = 8192)).buildJvmArgs()
        assertTrue(large.contains("-XX:+AlwaysPreTouch"))
    }

    @Test
    fun `custom GC policy leaves the user flags untouched`() {
        val root = tmpRoot()
        val user = listOf("-XX:+UseZGC", "-XX:ZCollectionInterval=5")
        val jvm = builder(root, gc = GcPolicy.Custom(user), extraJvm = user).buildJvmArgs()
        assertTrue(jvm.contains("-XX:+UseZGC"))
        assertFalse(jvm.contains("-XX:+UseG1GC"))
    }

    @Test
    fun `active processor count is pinned`() {
        val root = tmpRoot()
        val jvm = builder(root, activeProcessors = 6).buildJvmArgs()
        assertTrue(jvm.contains("-XX:ActiveProcessorCount=6"))
    }

    @Test
    fun `game args carry substituted auth placeholders`() {
        val root = tmpRoot()
        val game = builder(root).buildGameArgs()
        assertTrue(game.contains("basalt_test"))
        assertTrue(game.contains("11112222333344445555666677778888"))
        assertFalse(game.any { it.contains("\${auth_player_name}") })
        assertFalse(game.any { it.contains("\${") }, "no unresolved placeholders allowed: $game")
    }

    @Test
    fun `assets index id substitutes from layout`() {
        val root = tmpRoot()
        val game = builder(root).buildGameArgs()
        assertTrue(game.contains("34"))
    }

    @Test
    fun `plan command list is well formed`() {
        val root = tmpRoot()
        val plan = builder(root).build()
        val command = plan.toCommandList("/runtime/bin/java")
        assertEquals("/runtime/bin/java", command.first())
        assertTrue(command.contains(plan.wrapperClass!!))
        assertTrue(command.contains("net.minecraft.client.main.Main"))
        // game args come after the main class
        val mainIdx = command.indexOf("net.minecraft.client.main.Main")
        assertTrue(command.size > mainIdx + 1)
    }

    @Test
    fun `child process exec returns the real exit code`() {
        val root = tmpRoot()
        val plan = LaunchPlan(
            jvmArgs = listOf("-version"),
            gameArgs = emptyList(),
            mainClass = "",
            classpath = emptyList(),
            classpathString = "",
            wrapperClass = null
        )
        val code = ChildProcessExec().exec(
            plan = plan,
            javaBinary = System.getProperty("java.home") + "/bin/java",
            outputSink = { }
        )
        // `java -version` exits 0
        assertEquals(0, code)
    }

    @Test
    fun `classpath artifact respects platform`() {
        val linuxOnly = LibraryEntry(
            name = "natives-lib",
            downloads = dev.leonxlnx.basalt.engine.model.LibraryDownloads(
                artifact = dev.leonxlnx.basalt.engine.model.DownloadArtifact(
                    path = "com/test/natives-linux.jar"
                )
            ),
            rules = listOf(
                LibraryRule(action = "allow", os = LibraryRule.RuleOs("linux"))
            )
        )
        val arm = dev.leonxlnx.basalt.engine.model.DownloadArtifact(
            path = "com/test/natives-linux-arm64.jar"
        )
        val armOnly = LibraryEntry(
            name = "natives-arm",
            downloads = dev.leonxlnx.basalt.engine.model.LibraryDownloads(artifact = arm),
            rules = listOf(
                LibraryRule(action = "allow", os = LibraryRule.RuleOs("linux", "arm64"))
            )
        )

        val x86 = ClasspathBuilder.classpathArtifact(linuxOnly, Platform.LINUX_X86_64)
        assertEquals("com/test/natives-linux.jar", x86?.path)
        assertEquals(null, ClasspathBuilder.classpathArtifact(armOnly, Platform.LINUX_X86_64))
        assertEquals(arm, ClasspathBuilder.classpathArtifact(armOnly, Platform.LINUX_ARM64))
    }
}

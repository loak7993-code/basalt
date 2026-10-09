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
import dev.leonxlnx.basalt.engine.model.MemoryPolicy
import dev.leonxlnx.basalt.engine.model.Platform
import dev.leonxlnx.basalt.engine.model.VersionManifest
import dev.leonxlnx.basalt.engine.model.applyHeapTo
import dev.leonxlnx.basalt.engine.model.applyTo
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonPrimitive
import java.io.File

/**
 * Assembles the complete launch argument plan from a parsed version manifest.
 *
 * Sources, in order:
 *  1. manifest `arguments.jvm` and `arguments.default-user-jvm` (rule-evaluated)
 *  2. the logging config argument when the manifest carries one
 *  3. caller extras (user JVM args)
 *  4. memory flags from [MemoryPolicy], replacing any manifest-provided heap flags
 *  5. GC flags from [GcPolicy], replacing any manifest-provided collector selection
 *
 * Game arguments follow the same rule evaluation, then placeholder substitution.
 */
class JvmArgsBuilder(
    private val manifest: VersionManifest,
    private val platform: Platform,
    private val layout: InstallLayout,
    private val memory: MemoryPolicy? = null,
    private val gc: GcPolicy? = null,
    private val auth: AuthContext,
    private val versionName: String,
    private val versionType: String = "release",
    private val features: LibraryFilter.Features = LibraryFilter.Features(),
    private val extraJvmArgs: List<String> = emptyList(),
    private val extraGameArgs: List<String> = emptyList(),
    private val logConfigPath: File? = null,
    private val activeProcessors: Int? = null,
    private val quickPlay: QuickPlay? = null
) {

    data class AuthContext(
        val playerName: String,
        val uuid: String,
        val accessToken: String = "0",
        val userType: String = "msa",
        val clientId: String = "",
        val xuid: String = "",
        val sessionId: String = ""
    )

    sealed interface QuickPlay {
        data class Singleplayer(val saveName: String) : QuickPlay
        data class Server(val address: String, val port: Int? = null) : QuickPlay
    }

    fun build(): LaunchPlan {
        val jvmArgs = buildJvmArgs()
        val gameArgs = buildGameArgs()
        val classpath = ClasspathBuilder.build(manifest.libraries, platform, layout, features)
        return LaunchPlan(
            jvmArgs = jvmArgs,
            gameArgs = gameArgs,
            mainClass = manifest.mainClass,
            classpath = classpath,
            classpathString = ClasspathBuilder.join(classpath),
            wrapperClass = WRAPPER_CLASS,
            requiredJavaMajor = manifest.javaVersion?.majorVersion,
            logConfigSha1 = manifest.logging?.get("client")?.file?.sha1
        )
    }

    fun buildJvmArgs(): List<String> {
        val args = mutableListOf<String>()

        // 1. manifest-provided JVM args + Mojang's default-user-jvm, rule-evaluated
        manifest.arguments?.jvm?.let { args += evaluatedEntries(it) }
        manifest.arguments?.defaultUserJvm?.let { args += evaluatedEntries(it) }

        // 2. logging config
        logConfigPath?.let { file ->
            manifest.logging?.get("client")?.argument?.let { argument ->
                args.add(argument.replace("\${path}", file.absolutePath))
            }
        }

        // 3. caller extras
        args += extraJvmArgs

        // 4. memory + GC through the shared policy appliers (the app's launcher
        // uses the exact same helpers, so plan and runtime cannot drift).
        if (memory != null) {
            memory.applyHeapTo(args)
            if (memory.effectiveHeapMb < ALWAYS_PRE_TOUCH_MIN_HEAP_MB) {
                args.removeIf { it == "-XX:+AlwaysPreTouch" }
            }
        }
        gc?.applyTo(args)

        // 6. pinned processor count (Android big.LITTLE reports both clusters)
        activeProcessors?.let { args.add("-XX:ActiveProcessorCount=$it") }

        return args
    }

    fun buildGameArgs(): List<String> {
        val args = mutableListOf<String>()

        // Pre-1.13 flat argument string
        manifest.minecraftArguments?.let { flat ->
            args += flat.split(" ").filter { it.isNotBlank() }.map { substitute(it) }
            return finishGameArgs(args)
        }

        manifest.arguments?.game?.let { args += evaluatedEntries(it) }
        return finishGameArgs(args)
    }

    private fun finishGameArgs(args: MutableList<String>): List<String> {
        quickPlay?.let { qp ->
            when (qp) {
                is QuickPlay.Singleplayer -> args.addAll(
                    listOf("--quickPlaySingleplayer", qp.saveName)
                )
                is QuickPlay.Server -> {
                    val port = qp.port ?: 25565
                    args.addAll(listOf("--quickPlayMultiplayer", "${qp.address}:$port"))
                }
            }
        }
        args += extraGameArgs
        return args
    }

    /** Rule-evaluates a manifest argument entry list and flattens values. */
    private fun evaluatedEntries(entries: List<JsonElement>): List<String> {
        val out = mutableListOf<String>()
        for (entry in entries) {
            when (entry) {
                is JsonPrimitive -> out.add(substitute(entry.content))
                is JsonObject -> {
                    val rules = entry.rules()
                    if (rules != null && !rulesAllowed(rules)) continue
                    when (val value = entry["value"]) {
                        is JsonPrimitive -> out.add(substitute(value.jsonPrimitive.content))
                        is JsonArray -> value.forEach { el ->
                            out.add(substitute(el.jsonPrimitive.content))
                        }
                        else -> Unit
                    }
                }
                else -> Unit
            }
        }
        return out
    }

    private fun JsonObject.rules(): List<dev.leonxlnx.basalt.engine.model.LibraryRule>? =
        this["rules"]?.let { raw ->
            VersionManifest.manifestJson.decodeFromString(
                kotlinx.serialization.builtins.ListSerializer(
                    dev.leonxlnx.basalt.engine.model.LibraryRule.serializer()
                ),
                raw.toString()
            )
        }

    private fun rulesAllowed(rules: List<dev.leonxlnx.basalt.engine.model.LibraryRule>): Boolean {
        var anyAllow = false
        for (rule in rules) {
            val matches = LibraryFilter.ruleMatches(rule, platform, features)
            if (matches && rule.action.equals("allow", true)) anyAllow = true
            if (matches && rule.action.equals("disallow", true)) return false
        }
        return anyAllow
    }

    private fun substitute(raw: String): String = raw
        .replace("\${auth_player_name}", auth.playerName)
        .replace("\${auth_uuid}", auth.uuid)
        .replace("\${auth_access_token}", auth.accessToken)
        .replace("\${user_type}", auth.userType)
        .replace("\${clientid}", auth.clientId)
        .replace("\${auth_xuid}", auth.xuid)
        .replace("\${auth_session_id}", auth.sessionId)
        .replace("\${version_name}", versionName)
        .replace("\${version_type}", versionType)
        .replace("\${game_directory}", layout.gameDir.absolutePath)
        .replace("\${assets_root}", layout.assetsDir.absolutePath)
        .replace("\${assets_index_name}", layout.assetIndexId)
        .replace("\${natives_directory}", layout.nativesDir.absolutePath)
        .replace("\${library_directory}", layout.librariesDir.absolutePath)
        .replace("\${classpath}", ClasspathBuilder.build(manifest.libraries, platform, layout, features).joinToString(":"))
        .replace("\${classpath_separator}", ":")
        .replace("\${resolution_width}", features.resolutionWidth?.toString() ?: "854")
        .replace("\${resolution_height}", features.resolutionHeight?.toString() ?: "480")
        .replace("\${os_name}", platform.osName)
        .replace("\${os_arch}", platform.archName)
        .replace("\${user_home}", layout.gameDir.absolutePath)

    private companion object {
        const val WRAPPER_CLASS = "mio.Wrapper"
        const val ALWAYS_PRE_TOUCH_MIN_HEAP_MB = 4096
    }
}

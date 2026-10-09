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

import dev.leonxlnx.basalt.engine.model.DownloadArtifact
import dev.leonxlnx.basalt.engine.model.LibraryEntry
import dev.leonxlnx.basalt.engine.model.Platform
import java.io.File

/**
 * Builds the game classpath for a planned launch.
 *
 * Layout contract (relative to [InstallLayout.librariesDir]): each artifact's
 * `path` from the manifest, exactly as Mojang lays them out on disk. The client
 * jar is prepended. Entries are joined with ':' (the Linux classpath separator).
 */
data class InstallLayout(
    val clientJar: File,
    val librariesDir: File,
    val nativesDir: File,
    val gameDir: File,
    val assetsDir: File,
    val assetIndexId: String,
    val runtimeHome: File,
    val lwjglLibraryPath: String? = null
)

object ClasspathBuilder {

    /**
     * The artifact a library contributes to the classpath on [platform].
     * A missing client jar is a launch-blocking preflight problem, not a
     * silent omission, so the client jar is always part of the plan.
     */
    fun classpathArtifact(
        library: LibraryEntry,
        platform: Platform,
        features: LibraryFilter.Features = LibraryFilter.Features()
    ): DownloadArtifact? {
        if (!LibraryFilter.isAllowed(library, platform, features)) return null
        return library.downloads?.artifact?.takeIf { !it.path.isNullOrBlank() }
    }

    fun build(
        libraries: List<LibraryEntry>,
        platform: Platform,
        layout: InstallLayout,
        features: LibraryFilter.Features = LibraryFilter.Features()
    ): List<String> {
        val entries = mutableListOf<String>()
        entries.add(layout.clientJar.absolutePath)
        for (library in libraries) {
            val artifact = classpathArtifact(library, platform, features) ?: continue
            entries.add(File(layout.librariesDir, artifact.path!!).absolutePath)
        }
        return entries
    }

    fun join(entries: List<String>): String = entries.joinToString(":")
}

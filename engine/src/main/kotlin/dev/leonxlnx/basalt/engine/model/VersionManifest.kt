/*
 * Basalt Engine
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

package dev.leonxlnx.basalt.engine.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement

/**
 * The slice of Mojang's version manifest the engine needs to plan a launch.
 * Only the fields a launcher actually consumes; unknown fields are ignored so
 * new manifest versions keep parsing.
 */
@Serializable
data class VersionManifest(
    val id: String,
    val mainClass: String,
    val assets: String? = null,
    val assetIndex: AssetIndex? = null,
    val javaVersion: JavaRequirement? = null,
    val downloads: Map<String, DownloadEntry> = emptyMap(),
    val libraries: List<LibraryEntry> = emptyList(),
    val arguments: ManifestArguments? = null,
    val logging: Map<String, LoggingConfig>? = null,
    /** Pre-1.13 versions carry a flat argument string instead of [arguments]. */
    val minecraftArguments: String? = null,
    val type: String? = null,
    val releaseTime: String? = null
) {
    @Serializable
    data class AssetIndex(val id: String, val url: String, val totalSize: Long = 0)

    @Serializable
    data class JavaRequirement(val component: String, val majorVersion: Int)

    @Serializable
    data class DownloadEntry(val url: String, val sha1: String? = null, val size: Long = 0)

    @Serializable
    data class LoggingConfig(
        val argument: String,
        val file: DownloadEntry? = null
    )

    companion object {
        internal val manifestJson = Json {
            ignoreUnknownKeys = true
            explicitNulls = false
        }

        fun parse(json: String): VersionManifest = manifestJson.decodeFromString(serializer(), json)
    }
}

@Serializable
data class LibraryEntry(
    val name: String? = null,
    val downloads: LibraryDownloads? = null,
    val rules: List<LibraryRule> = emptyList(),
    val url: String? = null,
    val natives: Map<String, String>? = null,
    val extract: JsonElement? = null
)

@Serializable
data class LibraryDownloads(
    val artifact: DownloadArtifact? = null,
    val classifiers: Map<String, DownloadArtifact> = emptyMap()
)

@Serializable
data class DownloadArtifact(
    val path: String? = null,
    val url: String? = null,
    val sha1: String? = null,
    val size: Long = 0
)

@Serializable
data class LibraryRule(
    val action: String,
    val os: RuleOs? = null,
    val features: Map<String, Boolean>? = null
) {
    @Serializable
    data class RuleOs(
        val name: String? = null,
        val arch: String? = null
    )
}

/**
 * Manifest argument entries: plain strings, or objects with their own rules and
 * a string or list value.
 */
@Serializable
data class ManifestArguments(
    val game: List<JsonElement> = emptyList(),
    val jvm: List<JsonElement> = emptyList(),
    /** Mojang's recommended JVM defaults for this version, platform-rule-gated. */
    @SerialName("default-user-jvm")
    val defaultUserJvm: List<JsonElement> = emptyList()
)

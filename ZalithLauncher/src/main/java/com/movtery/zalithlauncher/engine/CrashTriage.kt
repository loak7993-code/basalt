/*
 * Basalt crash triage (Rust engine bridge)
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

package com.movtery.zalithlauncher.engine

import com.movtery.zalithlauncher.utils.logging.Logger
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/** What went wrong with a launch, and what Basalt can do about it. */
data class CrashDiagnosis(
    /** Stable id, e.g. `heap_exhausted`. `unknown` when nothing matched. */
    val kind: String,
    /** One line for the player, e.g. "The game ran out of memory". */
    val headline: String,
    val exitCode: Int,
    /** What Basalt would change before retrying, in words. */
    val fixDescription: String,
    /** The machine-readable fix: [Fix.Type] plus its fields. */
    val fix: Fix,
    /** Log lines that triggered the match. */
    val evidence: List<String>,
    /** Tail of the log, so a failure always shows something. */
    val tail: List<String>,
) {
    /** True when Basalt knows how to fix this without a human. */
    val actionable: Boolean get() = fix.type != Fix.Type.NONE

    /** One line combining the headline and the fix, ready for a toast. */
    val summary: String get() = "$headline. ${fixDescription.trimEnd('.')}."

    sealed interface Fix {
        val description: String

        val type: Type

        enum class Type { SET_HEAP, SET_RENDERER, USE_JAVA, REFETCH, DISABLE_MODS, NONE }

        data class SetHeap(val heapMb: Int?) : Fix {
            override val type = Type.SET_HEAP
            override val description: String
                get() = heapMb?.let { "Retry with a $it MB heap" } ?: "Retry with an engine-chosen heap"
        }

        data class SetRenderer(val renderer: String) : Fix {
            override val type = Type.SET_RENDERER
            override val description: String get() = "Retry with the $renderer renderer"
        }

        data class UseJava(val major: Int) : Fix {
            override val type = Type.USE_JAVA
            override val description: String get() = "Retry on Java $major"
        }

        data class Refetch(val paths: List<String>) : Fix {
            override val type = Type.REFETCH
            override val description: String get() = "Re-download ${paths.size} file(s) and retry"
        }

        data class DisableMods(val files: List<String>) : Fix {
            override val type = Type.DISABLE_MODS
            override val description: String get() = "Move ${files.size} mod file(s) aside and retry"
        }

        data object None : Fix {
            override val type = Type.NONE
            override val description: String get() = "No automatic fix"
        }
    }
}

/**
 * Classifies a failed launch. The rules live in the Rust engine, next to the
 * rest of the launch policy, so Android and the Linux CLI agree on what a
 * given log means.
 */
object CrashTriage {
    private const val TAG = "CrashTriage"

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * @param logLines the game's log, in order. The engine keeps what it needs.
     * @param exitCode what the game process returned.
     * @param heapMb the heap the launch used.
     * @param deviceRamMb physical RAM, used to keep a heap fix in budget.
     * @param renderer the renderer the launch used, or null on desktop.
     * @param javaMajor the Java that ran the launch.
     * @param modsDir the instance mods directory, so a named mod can be found.
     */
    fun diagnose(
        logLines: List<String>,
        exitCode: Int,
        heapMb: Int? = null,
        deviceRamMb: Int? = null,
        renderer: String? = null,
        javaMajor: Int? = null,
        modsDir: String? = null,
    ): CrashDiagnosis? {
        val facts = buildString {
            append('{')
            append("\"heapMb\":").append(heapMb ?: "null")
            append(",\"deviceRamMb\":").append(deviceRamMb ?: "null")
            append(",\"renderer\":").append(renderer?.let { "\"$it\"" } ?: "null")
            append(",\"javaMajor\":").append(javaMajor ?: "null")
            append(",\"modsDir\":").append(modsDir?.let { "\"$it\"" } ?: "null")
            append('}')
        }
        val payload = json.encodeToString(logLines)
        return try {
            parse(RustEngine.diagnoseCrash(payload, exitCode, facts))
        } catch (th: Throwable) {
            // Triage is a convenience: a failure here must never mask the crash.
            Logger.warning(TAG, "Rust crash diagnosis unavailable", th)
            null
        }
    }

    private fun parse(raw: String): CrashDiagnosis? {
        val root = json.parseToJsonElement(raw).jsonObject
        if (root["error"] != null) return null
        val fixObject = root["fix"]?.jsonObject ?: return null
        val fix = when (fixObject["type"]?.jsonPrimitive?.contentOrNull) {
            "setHeap" -> CrashDiagnosis.Fix.SetHeap(fixObject["heapMb"]?.jsonPrimitive?.intOrNull)
            "setRenderer" -> CrashDiagnosis.Fix.SetRenderer(
                fixObject["renderer"]?.jsonPrimitive?.contentOrNull.orEmpty()
            )
            "useJava" -> CrashDiagnosis.Fix.UseJava(
                fixObject["major"]?.jsonPrimitive?.intOrNull ?: 0
            )
            "refetch" -> CrashDiagnosis.Fix.Refetch(strings(fixObject))
            "disableMods" -> CrashDiagnosis.Fix.DisableMods(strings(fixObject))
            else -> CrashDiagnosis.Fix.None
        }
        return CrashDiagnosis(
            kind = root["kind"]?.jsonPrimitive?.contentOrNull ?: "unknown",
            headline = root["headline"]?.jsonPrimitive?.contentOrNull ?: "The launch failed",
            exitCode = root["exitCode"]?.jsonPrimitive?.intOrNull ?: -1,
            fixDescription = fix.description,
            fix = fix,
            evidence = strings(root),
            tail = root["tail"]?.jsonArray?.map { it.jsonPrimitive.contentOrNull.orEmpty() }
                .orEmpty(),
        )
    }

    private fun strings(source: JsonObject): List<String> =
        source.jsonArrayOrNull("files")
            ?: source.jsonArrayOrNull("paths")
            ?: source.jsonArrayOrNull("evidence")
            ?: emptyList()

    private fun JsonObject.jsonArrayOrNull(key: String): List<String>? =
        this[key]?.takeIf { it is kotlinx.serialization.json.JsonArray }
            ?.jsonArray?.mapNotNull { it.jsonPrimitive.contentOrNull }
}

/** Reads a JSON flag, used by callers that gate triage on a setting. */
internal fun JsonObject.flag(key: String): Boolean = this[key]?.jsonPrimitive?.booleanOrNull == true
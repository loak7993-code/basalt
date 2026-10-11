/*
 * Basalt Engine (Rust bridge)
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

package com.movtery.zalithlauncher.engine

import android.util.Log
import com.movtery.zalithlauncher.utils.logging.Logger
import dev.leonxlnx.basalt.engine.model.applyTo
import dev.leonxlnx.basalt.engine.model.gcPolicyFromArgs
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive

/**
 * Bridge to the Rust engine (`libbasalt_engine.so`, crate `engine-rs`).
 *
 * The Rust engine is the runtime source of truth for launch policies. If the
 * native library cannot be loaded (an ABI or packaging problem), every call
 * degrades to the equivalent Kotlin logic and the failure is logged once, so a
 * broken build degrades to 1.2.0 behavior instead of crashing.
 */
object RustEngine {
    private const val TAG = "RustEngine"

    private val loaded: Boolean = try {
        System.loadLibrary("basalt_engine")
        Log.i(TAG, "libbasalt_engine.so loaded")
        true
    } catch (th: Throwable) {
        Log.e(TAG, "loadLibrary failed", th)
        Logger.warning(TAG, "Rust engine unavailable, falling back to Kotlin policies", th)
        false
    }

    /** Diagnostics: parses a manifest through the Rust engine. */
    fun probe(manifestJson: String): String = nativeProbe(manifestJson)

    private external fun nativeProbe(manifestJson: String): String

    /** The device-proportional memory default, computed by the Rust engine. */
    fun defaultRamAllocation(deviceRamMb: Int, is32Bit: Boolean): Int {
        if (loaded) {
            return nativeDefaultRamAllocation(deviceRamMb, if (is32Bit) 1 else 0)
        }
        return kotlinFallbackDefaultRam(deviceRamMb, is32Bit)
    }

    /**
     * Applies the engine GC policy (tuned G1 baseline unless a user collector
     * is present) to the arg list. Returns the same list object mutated.
     */
    fun applyGcPolicy(args: MutableList<String>): MutableList<String> {
        if (loaded) {
            try {
                val json = Json.encodeToString(
                    JsonArray.serializer(),
                    JsonArray(args.map { JsonPrimitive(it) })
                )
                val out = Json.decodeFromString(
                    JsonArray.serializer(),
                    nativeApplyGcPolicy(json)
                )
                args.clear()
                out.forEach { element ->
                    if (element is JsonPrimitive) args.add(element.content)
                }
                return args
            } catch (th: Throwable) {
                Logger.warning(TAG, "Rust applyGcPolicy failed, using Kotlin policy", th)
            }
        }
        gcPolicyFromArgs(args).applyTo(args)
        return args
    }

    /**
     * Classifies a failed launch. `linesJson` is a JSON array of log lines,
     * `factsJson` carries what the host knows about the launch. Returns a JSON
     * object describing the crash and the fix, or {error} when the call fails.
     */
    fun diagnoseCrash(linesJson: String, exitCode: Int, factsJson: String): String {
        if (loaded) return nativeDiagnoseCrash(linesJson, exitCode, factsJson)
        return "{\"error\":\"engine unavailable\"}"
    }

    private external fun nativeDiagnoseCrash(
        linesJson: String,
        exitCode: Int,
        factsJson: String,
    ): String

    private external fun nativeDefaultRamAllocation(deviceRamMb: Int, is32Bit: Int): Int
    private external fun nativeApplyGcPolicy(argsJson: String): String

    // Kotlin fallbacks, identical in behavior to the Rust implementation.

    private fun kotlinFallbackDefaultRam(deviceRamMb: Int, is32Bit: Boolean): Int {
        val ram = deviceRamMb.toDouble()
        val max: Double = if (is32Bit || ram < 2048.0) {
            1024.0.coerceAtMost(ram)
        } else {
            ram - if (ram < 3064.0) 800.0 else 1024.0
        }
        val target = (ram * 0.55).toInt()
        val floor = 1536.coerceAtMost(max.toInt())
        return target.coerceAtLeast(floor).coerceAtMost(max.toInt())
    }
}

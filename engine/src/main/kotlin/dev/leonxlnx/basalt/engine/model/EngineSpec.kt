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

/**
 * The platform the engine plans for. Kept as an explicit enum so arg assembly
 * never keys off stringly-typed OS names.
 */
enum class Platform(val osName: String, val archName: String) {
    LINUX_ARM64("linux", "arm64"),
    LINUX_X86_64("linux", "x86_64");

    companion object {
        /** Maps raw system properties to a [Platform]; null when unsupported. */
        fun from(os: String?, arch: String?): Platform? = when {
            os.equals("linux", ignoreCase = true) -> when (arch) {
                "aarch64", "arm64" -> LINUX_ARM64
                "amd64", "x86_64" -> LINUX_X86_64
                else -> null
            }
            else -> null
        }
    }
}

/**
 * Memory policy for the game JVM.
 *
 * @param heapMb the -Xms/-Xmx value in MB. The device default is computed by
 * the caller (app side reads ActivityManager); the engine only clamps it into
 * a sane window so a bad reading can never produce a 256 MB heap on a device
 * that supports more, or a heap larger than the device can give.
 */
data class MemoryPolicy(
    val heapMb: Int,
    val minMb: Int = 512,
    val maxMb: Int = 8192
) {
    val effectiveHeapMb: Int = heapMb.coerceIn(minMb, maxMb)
}

/**
 * Garbage collection policy. [tunedG1] is the phone-class baseline: keeps
 * frame-time spikes down under constrained heaps. [none] leaves the JVM
 * default. [custom] means the user already picked flags; the engine must not
 * inject anything.
 */
sealed interface GcPolicy {
    data object TunedG1 : GcPolicy
    data object None : GcPolicy

    /** @param flags the user's own -XX GC flags, passed through untouched. */
    data class Custom(val flags: List<String>) : GcPolicy

    companion object {
        /**
         * Picks the policy from the final JVM arg list: any user-selected GC
         * wins, otherwise the tuned baseline applies.
         */
        fun fromArgs(args: List<String>): GcPolicy {
            val userGc = args.filter { it.startsWith("-XX:+Use") && it.endsWith("GC") }
            return when {
                userGc.isNotEmpty() -> Custom(args.filter { it.startsWith("-XX:") })
                args.any { it.startsWith("-XX:+UseG1GC") } -> TunedG1
                else -> TunedG1
            }
        }
    }
}

/** The tuned G1 baseline flags for phone-class heaps. */
val TunedG1Flags: List<String> = listOf(
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1NewSizePercent=20",
    "-XX:G1ReservePercent=20",
    "-XX:MaxGCPauseMillis=40",
    "-XX:G1HeapRegionSize=16M",
    "-XX:+DisableExplicitGC"
)

/**
 * Flags that a GC/memory policy owns when it overrides manifest defaults:
 * collector selection and its tuning knobs. Object-layout flags such as
 * -XX:+UseCompactObjectHeaders are NOT owned by the GC policy and survive.
 */
val GcOwnedFlagPrefixes: List<String> = listOf(
    "-XX:+UseG1GC", "-XX:+UseZGC", "-XX:+UseSerialGC", "-XX:+UseParallelGC",
    "-XX:+UseParallelOldGC", "-XX:+UseConcMarkSweepGC", "-XX:+UseShenandoahGC",
    "-XX:+UnlockExperimentalVMOptions", "-XX:+DisableExplicitGC",
    "-XX:G1NewSizePercent=", "-XX:G1MaxNewSizePercent=", "-XX:G1ReservePercent=",
    "-XX:MaxGCPauseMillis=", "-XX:G1HeapRegionSize=", "-XX:G1HeapWastePercent=",
    "-XX:G1MixedGCCountTarget=", "-XX:InitiatingHeapOccupancyPercent="
)

/** Flags the heap policy owns: the launcher decides memory, not the manifest. */
val HeapOwnedFlagPrefixes: List<String> = listOf("-Xms", "-Xmx")

/** Applies heap flags to [args], replacing anything a manifest provided. */
fun MemoryPolicy.applyHeapTo(args: MutableList<String>) {
    args.removeIf { arg -> HeapOwnedFlagPrefixes.any { arg.startsWith(it) } }
    args.add("-Xms${effectiveHeapMb}M")
    args.add("-Xmx${effectiveHeapMb}M")
}

/** Applies the collector policy to [args], replacing manifest-provided ones. */
fun GcPolicy.applyTo(args: MutableList<String>) {
    when (this) {
        is GcPolicy.TunedG1 -> {
            args.removeIf { arg -> GcOwnedFlagPrefixes.any { arg.startsWith(it) } }
            args.addAll(TunedG1Flags)
        }
        is GcPolicy.None -> Unit
        is GcPolicy.Custom -> Unit
    }
}

/**
 * Picks the policy from an existing arg list: any user-selected GC wins,
 * otherwise the tuned baseline applies.
 */
fun gcPolicyFromArgs(args: List<String>): GcPolicy =
    if (args.any { it.startsWith("-XX:+Use") && it.endsWith("GC") }) {
        GcPolicy.Custom(args.filter { it.startsWith("-XX:") })
    } else {
        GcPolicy.TunedG1
    }

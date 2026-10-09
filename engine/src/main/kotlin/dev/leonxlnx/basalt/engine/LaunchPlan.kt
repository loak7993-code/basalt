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

/**
 * The immutable result of a launch plan: everything needed to start the game
 * JVM, in execution order, plus the pieces the host app needs for diagnostics.
 */
data class LaunchPlan(
    /** JVM arguments, in order: manifest defaults, logging, extras, memory, GC. */
    val jvmArgs: List<String>,
    /** Game arguments after the main class. */
    val gameArgs: List<String>,
    val mainClass: String,
    /** Classpath entries in classloader order. */
    val classpath: List<String>,
    val classpathString: String,
    /**
     * The launcher bridge class the host app prepends so its window/input
     * layer can hook the game. Null when launching bare.
     */
    val wrapperClass: String? = null,
    /** The manifest's required Java major version, when declared. */
    val requiredJavaMajor: Int? = null,
    /** The log4j config the manifest expects, for download verification. */
    val logConfigSha1: String? = null
) {
    /**
     * The full exec list: java binary + jvm args + (wrapper, main class) + game args.
     */
    fun toCommandList(javaBinary: String): List<String> =
        buildList {
            add(javaBinary)
            addAll(jvmArgs)
            if (!wrapperClass.isNullOrBlank()) {
                add(wrapperClass)
                add(mainClass)
            } else {
                add(mainClass)
            }
            addAll(gameArgs)
        }
}

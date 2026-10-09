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

import java.io.File

/**
 * How the engine starts the planned JVM.
 *
 * [ChildProcessExec] spawns a real java process and streams its output to the
 * sink; it is the reference implementation and runs on any desktop JVM, which
 * is what the engine's tests use.
 *
 * An in-process exec (Android hosts load the bionic JVM into the app process
 * through a native bridge) implements the same interface; the host app binds
 * that adapter.
 */
interface ExecStrategy {
    /**
     * @param outputSink receives each line the JVM prints, in order.
     * @return the process exit code.
     */
    fun exec(plan: LaunchPlan, javaBinary: String, outputSink: (String) -> Unit): Int

    fun exec(plan: LaunchPlan, javaBinary: String, outputSink: File): Int =
        exec(plan, javaBinary) { line -> outputSink.appendText(line + System.lineSeparator()) }
}

/**
 * Reference exec: a child java process. Used by tests and by any host that can
 * spawn processes directly.
 */
class ChildProcessExec : ExecStrategy {
    override fun exec(plan: LaunchPlan, javaBinary: String, outputSink: (String) -> Unit): Int {
        val command = plan.toCommandList(javaBinary)
        val process = ProcessBuilder(command)
            .redirectErrorStream(true)
            .start()
        process.inputStream.bufferedReader().useLines { lines ->
            lines.forEach(outputSink)
        }
        return process.waitFor()
    }
}

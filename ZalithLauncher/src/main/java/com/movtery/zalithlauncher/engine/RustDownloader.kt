/*
 * Basalt Rust downloader bridge
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

package com.movtery.zalithlauncher.engine

import android.util.Log
import com.movtery.zalithlauncher.game.download.engine.DownloadRequest
import com.movtery.zalithlauncher.game.version.download.DownloadTask
import com.movtery.zalithlauncher.utils.logging.Logger
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import kotlinx.serialization.json.put

/**
 * Bridge to the Rust download engine: shared keep-alive agent, bounded worker
 * threads, 60s timeouts, 3-attempt retries with candidate rotation, SHA1
 * verification, and byte-accurate progress. Job-based: start, poll, wait.
 */
object RustDownloader {
    private const val TAG = "RustDownloader"

    val available: Boolean = try {
        System.loadLibrary("basalt_engine")
        Logger.info(TAG, "Rust downloader active")
        true
    } catch (th: Throwable) {
        Logger.warning(TAG, "Rust downloader unavailable; using the Kotlin engine", th)
        false
    }

    @Serializable
    data class Snapshot(
        val done: Int,
        val total: Int,
        val bytesDone: Long,
        val bytesTotal: Long,
        val finished: Boolean
    )

    @Serializable
    data class Failure(val path: String, val error: String)

    private val json = Json { ignoreUnknownKeys = true }

    private fun tasksToJson(tasks: List<DownloadTask>): String {
        val array = JsonArray(
            tasks.map { task ->
                JsonObject(
                    mapOf(
                        "urls" to JsonArray(task.urls.map { JsonPrimitive(it) }),
                        "target" to JsonPrimitive(task.targetFile.absolutePath),
                        "sha1" to (task.sha1?.let { JsonPrimitive(it) } ?: JsonPrimitive(null as String?)),
                        "size" to JsonPrimitive(task.size)
                    )
                )
            }
        )
        return array.toString()
    }

    /** Starts a Rust download job for the given tasks. Returns the job id. */
    fun start(requests: List<DownloadRequest>, workers: Int): Int {
        val array = JsonArray(
            requests.map { request ->
                buildJsonObject {
                    put(
                        "urls",
                        JsonArray(request.urls.map { JsonPrimitive(it) })
                    )
                    put("target", request.targetFile.absolutePath)
                    request.sha1?.let { put("sha1", it) }
                    if (request.expectedSize > 0) put("size", request.expectedSize)
                }
            }
        )
        return nativeDownloadStart(array.toString(), workers)
    }

    fun progress(jobId: Int): Snapshot? {
        val raw = nativeDownloadProgress(jobId)
        return runCatching { json.decodeFromString(Snapshot.serializer(), raw) }.getOrNull()
    }

    /** Blocks until the job finishes. Returns failures keyed by path. */
    fun wait(jobId: Int): Map<String, String> {
        val raw = nativeDownloadWait(jobId)
        val failures = json.decodeFromString<List<Failure>>(raw)
        return failures.associate { it.path to it.error }
    }

    private external fun nativeDownloadStart(tasksJson: String, workers: Int): Int
    private external fun nativeDownloadProgress(jobId: Int): String
    private external fun nativeDownloadWait(jobId: Int): String
}

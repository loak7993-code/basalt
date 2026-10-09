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

import dev.leonxlnx.basalt.engine.model.LibraryEntry
import dev.leonxlnx.basalt.engine.model.LibraryRule
import dev.leonxlnx.basalt.engine.model.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class LibraryFilterTest {

    private fun rule(action: String, osName: String? = null, arch: String? = null) =
        LibraryEntry(
            name = "test",
            rules = listOf(LibraryRule(action = action, os = LibraryRule.RuleOs(osName, arch)))
        )

    @Test
    fun `library without rules is allowed on every platform`() {
        val library = LibraryEntry(name = "plain")
        Platform.entries.forEach { platform ->
            assertTrue(LibraryFilter.isAllowed(library, platform))
        }
    }

    @Test
    fun `osx-only library is rejected on linux`() {
        val library = rule("allow", osName = "osx")
        assertFalse(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64))
        assertTrue(LibraryFilter.isAllowed(library, Platform.LINUX_ARM64).not())
    }

    @Test
    fun `linux allow rule passes for both linux arches`() {
        val library = rule("allow", osName = "linux")
        assertTrue(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64))
        assertTrue(LibraryFilter.isAllowed(library, Platform.LINUX_ARM64))
    }

    @Test
    fun `arch restriction is enforced`() {
        val armOnly = rule("allow", osName = "linux", arch = "arm64")
        assertTrue(LibraryFilter.isAllowed(armOnly, Platform.LINUX_ARM64))
        assertFalse(LibraryFilter.isAllowed(armOnly, Platform.LINUX_X86_64))
    }

    @Test
    fun `disallow wins over allow`() {
        val library = LibraryEntry(
            name = "mixed",
            rules = listOf(
                LibraryRule(action = "allow", os = LibraryRule.RuleOs("linux")),
                LibraryRule(action = "disallow", os = LibraryRule.RuleOs("linux"))
            )
        )
        assertFalse(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64))
    }

    @Test
    fun `known feature gates the rule`() {
        val library = LibraryEntry(
            name = "res",
            rules = listOf(
                LibraryRule(
                    action = "allow",
                    features = mapOf("has_custom_resolution" to true)
                )
            )
        )
        val plain = LibraryFilter.Features()
        val custom = LibraryFilter.Features(hasCustomResolution = true)
        assertFalse(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64, plain))
        assertTrue(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64, custom))
    }

    @Test
    fun `unknown feature is treated as false`() {
        val library = LibraryEntry(
            name = "x",
            rules = listOf(
                LibraryRule(action = "allow", features = mapOf("is_demo_user" to false))
            )
        )
        assertTrue(LibraryFilter.isAllowed(library, Platform.LINUX_X86_64))
    }

    @Test
    fun `platform mapping from raw properties`() {
        assertEquals(Platform.LINUX_ARM64, Platform.from("Linux", "aarch64"))
        assertEquals(Platform.LINUX_X86_64, Platform.from("Linux", "amd64"))
        assertEquals(null, Platform.from("windows", "amd64"))
        assertEquals(null, Platform.from("linux", "i386"))
    }
}

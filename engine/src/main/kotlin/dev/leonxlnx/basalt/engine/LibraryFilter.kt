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

/**
 * Evaluates Mojang library rules for a target platform.
 *
 * Rules are a conjunction: a rule matches when every condition it carries is
 * satisfied (os name, os arch, features). A library is allowed when no "allow"
 * rule exists (default allow), or when at least one allow rule matches and no
 * deny rule matches.
 */
object LibraryFilter {

    data class Features(
        val hasCustomResolution: Boolean = false,
        val resolutionWidth: Int? = null,
        val resolutionHeight: Int? = null,
        val isDemoUser: Boolean = false,
        val hasQuickPlaysSupport: Boolean = false,
        val isQuickPlaySingleplayer: Boolean = false,
        val isQuickPlayMultiplayer: Boolean = false,
        val isQuickPlayRealms: Boolean = false
    ) {
        fun named(name: String): Boolean? = when (name) {
            "has_custom_resolution" -> hasCustomResolution
            "is_demo_user" -> isDemoUser
            "has_quick_plays_support" -> hasQuickPlaysSupport
            "is_quick_play_singleplayer" -> isQuickPlaySingleplayer
            "is_quick_play_multiplayer" -> isQuickPlayMultiplayer
            "is_quick_play_realms" -> isQuickPlayRealms
            else -> null
        }
    }

    fun ruleMatches(rule: LibraryRule, platform: Platform, features: Features): Boolean {
        rule.os?.let { os ->
            os.name?.let { name ->
                if (!name.equals(platform.osName, ignoreCase = true)) return false
            }
            os.arch?.let { arch ->
                if (!arch.equals(platform.archName, ignoreCase = true)) return false
            }
        }
        rule.features?.forEach { (name, expected) ->
            val actual = features.named(name)
            // Unknown features are treated as false, matching launcher convention.
            if (actual != expected) return false
        }
        return true
    }

    fun isAllowed(
        library: LibraryEntry,
        platform: Platform,
        features: Features = Features()
    ): Boolean {
        if (library.rules.isEmpty()) return true
        var anyAllowMatch = false
        for (rule in library.rules) {
            val matches = ruleMatches(rule, platform, features)
            if (matches && rule.action.equals("allow", ignoreCase = true)) anyAllowMatch = true
            if (matches && rule.action.equals("disallow", ignoreCase = true)) return false
        }
        return anyAllowMatch
    }
}

/*
 * Zalith Launcher 2
 * Copyright (C) 2025 MovTery <movtery228@qq.com> and contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/gpl-3.0.txt>.
 */

package com.movtery.zalithlauncher.setting

import android.content.Context
import com.movtery.zalithlauncher.utils.platform.getDefaultRamAllocation
import com.movtery.zalithlauncher.utils.string.splitPreservingQuotes

private const val LWJGL_LIB_NAME_ARG = "-Dorg.lwjgl.opengl.libname="

/**
 * 初始化处理所有设置项
 * @param reloadAll 是否重新加载全部设置项
 */
fun loadAllSettings(context: Context, reloadAll: Boolean = false) {
    if (reloadAll) AllSettings.reloadAll()
    if (AllSettings.ramAllocation.getValue() == null) {
        val ram = getDefaultRamAllocation(context)
        AllSettings.ramAllocation.save(ram)
    }
    val jvmArgs = AllSettings.jvmArgs.getValue()
    jvmArgs.splitPreservingQuotes().find { it.startsWith(LWJGL_LIB_NAME_ARG) }?.let { arg ->
        AllSettings.jvmArgs.save(jvmArgs.replace(arg, ""))
    }
}
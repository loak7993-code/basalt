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

package com.movtery.zalithlauncher.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.sp
import com.movtery.zalithlauncher.R

/**
 * Geist Sans for all UI text, Geist Mono for numeric and code surfaces.
 * Both families are bundled in res/font (OFL license), never loaded from a CDN.
 */
val GeistSans = FontFamily(
    androidx.compose.ui.text.font.Font(R.font.geist_regular, FontWeight.Normal),
    androidx.compose.ui.text.font.Font(R.font.geist_medium, FontWeight.Medium),
    androidx.compose.ui.text.font.Font(R.font.geist_semibold, FontWeight.SemiBold),
    androidx.compose.ui.text.font.Font(R.font.geist_bold, FontWeight.Bold),
)

val GeistMono = FontFamily(
    androidx.compose.ui.text.font.Font(R.font.geist_mono_regular, FontWeight.Normal),
    androidx.compose.ui.text.font.Font(R.font.geist_mono_medium, FontWeight.Medium),
    androidx.compose.ui.text.font.Font(R.font.geist_mono_semibold, FontWeight.SemiBold),
)

private fun geist(
    fontSize: Int,
    lineHeight: Int,
    weight: FontWeight = FontWeight.Normal,
    letterSpacing: Double = 0.0
) = TextStyle(
    fontFamily = GeistSans,
    fontWeight = weight,
    fontSize = fontSize.sp,
    lineHeight = lineHeight.sp,
    letterSpacing = letterSpacing.sp
)

val AppTypography = Typography(
    displayLarge = geist(57, 64, FontWeight.SemiBold, -0.25),
    displayMedium = geist(45, 52, FontWeight.SemiBold, -0.25),
    displaySmall = geist(36, 44, FontWeight.SemiBold),
    headlineLarge = geist(32, 40, FontWeight.SemiBold),
    headlineMedium = geist(28, 36, FontWeight.SemiBold),
    headlineSmall = geist(24, 32, FontWeight.Medium),
    titleLarge = geist(22, 28, FontWeight.Medium),
    titleMedium = geist(16, 24, FontWeight.SemiBold, 0.15),
    titleSmall = geist(14, 20, FontWeight.Medium, 0.1),
    bodyLarge = geist(16, 24, FontWeight.Normal, 0.5),
    bodyMedium = geist(14, 20, FontWeight.Normal, 0.25),
    bodySmall = geist(12, 16, FontWeight.Normal, 0.4),
    labelLarge = geist(14, 20, FontWeight.Medium, 0.1),
    labelMedium = geist(12, 16, FontWeight.Medium, 0.5),
    labelSmall = geist(11, 16, FontWeight.Medium, 0.5)
)

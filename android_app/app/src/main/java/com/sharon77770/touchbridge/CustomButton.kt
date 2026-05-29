package com.sharon77770.touchbridge

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

private const val PREF_CUSTOM_BUTTONS = "custom_buttons"

data class CustomButton(
    val id: String,
    val label: String,
    val position: Int,
)

fun loadCustomButtons(context: Context): List<CustomButton> {
    val raw = context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .getString(PREF_CUSTOM_BUTTONS, null)
        ?: return emptyList()

    return runCatching {
        val array = JSONArray(raw)
        buildList {
            for (index in 0 until array.length()) {
                val item = array.getJSONObject(index)
                val id = item.optString("id").trim()
                val label = item.optString("label").trim()

                if (id.isNotBlank() && label.isNotBlank()) {
                    add(
                        CustomButton(
                            id = id,
                            label = label,
                            position = item.optInt("position", index),
                        ),
                    )
                }
            }
        }.normalizedCustomButtonPositions()
    }.getOrDefault(emptyList())
}

fun saveCustomButtons(context: Context, buttons: List<CustomButton>) {
    val array = JSONArray()
    buttons.normalizedCustomButtonPositions().forEach { button ->
        array.put(
            JSONObject()
                .put("id", button.id)
                .put("label", button.label)
                .put("position", button.position),
        )
    }

    context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putString(PREF_CUSTOM_BUTTONS, array.toString())
        .apply()
}

fun List<CustomButton>.normalizedCustomButtonPositions(): List<CustomButton> {
    return sortedWith(compareBy<CustomButton> { it.position }.thenBy { it.label })
        .mapIndexed { index, button -> button.copy(position = index) }
}

fun sanitizeCustomButtonId(raw: String): String {
    return raw
        .trim()
        .lowercase()
        .replace(Regex("[^a-z0-9_.-]+"), "_")
        .trim('_')
}

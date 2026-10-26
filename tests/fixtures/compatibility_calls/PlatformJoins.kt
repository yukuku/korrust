package compatibility.calls
import compatibility.calls.lib.CallEffects

fun platformKnown(id: String): String { CallEffects.mark(id, 0); return "known" }
fun platformMaybe(id: String, missing: Boolean): String? { CallEffects.mark(id, 0); return if (missing) null else "maybe" }
fun platformConsume(value: String, later: Int): String { CallEffects.mark("U", 0); return value + later }

fun platformFirstNullable(flag: Boolean, missing: Boolean): String? =
    if (CallEffects.condition("C", flag)) CallEffects.platform("P", missing) else platformKnown("K")
fun platformLastNullable(flag: Boolean, missing: Boolean): String? =
    if (CallEffects.condition("C", flag)) platformKnown("K") else CallEffects.platform("P", missing)
fun platformNullableFirst(flag: Boolean, platformMissing: Boolean, nullableMissing: Boolean): String? =
    if (CallEffects.condition("C", flag)) CallEffects.platform("P", platformMissing) else platformMaybe("N", nullableMissing)
fun nullablePlatformFirst(flag: Boolean, platformMissing: Boolean, nullableMissing: Boolean): String? =
    if (CallEffects.condition("C", flag)) platformMaybe("N", nullableMissing) else CallEffects.platform("P", platformMissing)

fun platformFirstRequired(flag: Boolean, missing: Boolean): String =
    platformConsume(if (CallEffects.condition("C", flag)) CallEffects.platform("P", missing) else platformKnown("K"), CallEffects.mark("L", 9))
fun platformLastRequired(flag: Boolean, missing: Boolean): String =
    platformConsume(if (CallEffects.condition("C", flag)) platformKnown("K") else CallEffects.platform("P", missing), CallEffects.mark("L", 9))
fun platformNullableAsserted(flag: Boolean, platformMissing: Boolean, nullableMissing: Boolean): String =
    platformConsume((if (CallEffects.condition("C", flag)) CallEffects.platform("P", platformMissing) else platformMaybe("N", nullableMissing))!!, CallEffects.mark("L", 9))
fun nullablePlatformAsserted(flag: Boolean, platformMissing: Boolean, nullableMissing: Boolean): String =
    platformConsume((if (CallEffects.condition("C", flag)) platformMaybe("N", nullableMissing) else CallEffects.platform("P", platformMissing))!!, CallEffects.mark("L", 9))

fun platformJoins(): String = platformFirstNullable(false, true) ?: "unexpected"

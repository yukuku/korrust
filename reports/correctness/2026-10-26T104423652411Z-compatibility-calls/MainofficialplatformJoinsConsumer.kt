fun main() {
    val first: String? = compatibility.calls.platformFirstNullable(true, true)
    val last: String? = compatibility.calls.platformLastNullable(false, true)
    val nullableFirst: String? = compatibility.calls.nullablePlatformFirst(true, false, true)
    val nullableLast: String? = compatibility.calls.platformNullableFirst(false, false, true)
    check(first == null && last == null && nullableFirst == null && nullableLast == null)
    val requiredFirst: String = compatibility.calls.platformFirstRequired(false, true)
    val requiredLast: String = compatibility.calls.platformLastRequired(true, true)
    check(requiredFirst == "known9" && requiredLast == "known9")
}

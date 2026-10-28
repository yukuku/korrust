package newfamilies.strings.consumer

import newfamilies.strings.*

fun check(): String {
    val text: String = templates("K", 2)
    val missing: Int? = safeNavigation(null)
    val present: Int? = safeNavigation("abc")
    val cast: String? = casting(3)
    if (text != "K:3:\\${'$'}name:${'$'}: ${'$'} 9") return "template"
    if (missing != null || present != 3 || cast != null) return "nullable"
    if (unicode().length != 5 || arithmetic() != 7) return "value"
    return "OK"
}

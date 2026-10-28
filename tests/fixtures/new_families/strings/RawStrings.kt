package newfamilies.strings

fun plain(): String = """\n\u0041\t\"""
fun unicode(): String = """😀é𝄞"""
fun quotes(): String = """""" + "|" + """"""" + "|" + """""""" + "|" + """"""""" + "|" + """" a """"
fun multiline(): String = """
  first
  second
"""
fun templates(name: String, n: Int): String = """$name:${n + 1}:\${'$'}name:${'$'}: $ 9"""
fun nested(): String = """${"a${"""b${"c"}"""}"}"""
fun expressionLines(): String = """${
    (1
    + 2)
}"""
fun ordinary(): String = "\n\u0041\t\\\"\$😀"
fun shortQuotes(): String = """one " two "" end"""
fun nestedComment(): String = """${(1 /* } /* nested */ */ + 2)}"""
fun trailingSlash(): String = """ends\"""
fun nullValue(value: String?): String = """[$value]"""
fun failure(): String { throw IllegalArgumentException("raw") }
fun throwing(): String = """${failure()}"""
fun ordered(): String {
    var value = 0
    fun next(): Int { value += 1; return value }
    return """${next()}:${next()}:${next()}"""
}
fun navigation(value: String): Int = value
    .substring(
        1
        + 1
    )
    .
    length
fun safeNavigation(value: String?): Int? = value
    ?.
    length
fun arithmetic(): Int = (1
    + 2 *
      3)
fun statementBoundary(): Int {
    val value = 1
    + 2
    return value
}
fun lambdaBoundary(): Int = invokeInt({
    val value = 1
    + 2
    value
})
fun invokeInt(action: () -> Int): Int = action()
fun indexing(values: IntArray): Int = values[
    1
    + 1
]
fun unary(): Int = -
    2
fun booleans(): Boolean = true
    && false
    || true
fun elvis(value: String?): String = value
    ?: "fallback"
fun casting(value: Any?): String? = value
    as? String
fun comments(value: String): Int = value /* outer
    /* inner */ end */
    // line
    .length
fun commentArithmetic(): Int = 1 /* outer
    /* inner */ end */ + 2
const val RAW_LITERAL: String = """\n\u0041"""
fun charTemplate(value: Char): String = """$value"""
fun nullableCharTemplate(value: Char?): String = """$value"""

fun ignore(): Int = 1
fun ignore(action: () -> Int): Int = 2
fun ignoredTrailingLambda(): Int {
    var value = 0
    ignore()
    { value = 3; 4 }
    return value
}
fun executedTrailingLambda(): Int = invokeInt()
    { 7 }
fun labeledTrailingLambda(): Int = invokeInt()
    marker@{ return@marker 8 }

fun <T> invokeGeneric(action: () -> T): T = action()
fun genericTrailingLambda(): String = invokeGeneric<String>
    { "generic" }

package newfamilies.constants

import newfamilies.library.JavaConstants
import newfamilies.library.JavaConstants.BASE as IMPORTED
import newfamilies.library.RuntimeConstant
import newfamilies.shared.EXTERNAL
import newfamilies.shared.INTERNAL

const val BASE = 2 * 21
const val PARENTHESIZED = (1)
const val LATER = 3
const val FORWARD = LATER + BASE
const val INT_WRAP = 2147483647 + 1
const val LONG_WRAP = 9223372036854775807L + 1L
const val INT_DIV_MIN = -2147483648 / -1
const val INT_REM_MIN = -2147483648 % -1
const val MIN_LONG = -9223372036854775807L - 1L
const val LONG_DIV_MIN = MIN_LONG / -1L
const val LONG_REM_MIN = MIN_LONG % -1L
const val DECLARED_LONG: Long = 1 + 2
const val INT_OVERFLOW_TO_LONG: Long = 2147483647 + 1
const val INT_DIV_TO_LONG: Long = -2147483648 / -1
const val MIXED_LONG = 7 + 5L * 3
const val FLOAT_RESULT = 1.25f + 2
const val DOUBLE_RESULT = 1.5 + 2L
const val FLOAT_NAN = 0.0f / 0.0f
const val DOUBLE_NAN = 0.0 / 0.0
const val FLOAT_INFINITY = 1.0f / 0.0f
const val DOUBLE_INFINITY = -1.0 / 0.0
const val FLOAT_NEGATIVE_ZERO = -0.0f + -0.0f
const val DOUBLE_NEGATIVE_ZERO = -0.0 * 1.0
const val NAN_EQUALITY = DOUBLE_NAN == DOUBLE_NAN
const val ZERO_EQUALITY = DOUBLE_NEGATIVE_ZERO == 0.0
const val UTF16_ORDER = "😀" < "\uE000"
const val CHAR_WRAP = '\uFFFF' + 1
const val CHAR_DISTANCE = 'z' - 'a'
const val CHARACTER = 'a' + 2147483647
const val TEXT = "a" + "b" + CHAR_DISTANCE
const val TEMPLATE = """x${BASE}:${'\uD800'}:${null}"""
const val TRUE_VALUE = (2 * 3 > 5) && !false
const val FALSE_LAZY = false && (1 / 0 > 0)
const val TRUE_LAZY = true || (1 / 0 > 0)
const val STRING_ORDER = "a" < "b"
const val BOOLEAN_ORDER = true > false
const val FLOAT_COMPARE = 16777217L < 16777216f
const val IMPORTED_VALUE = IMPORTED * 2 + JavaConstants.BASE
const val IMPORTED_WIDE = JavaConstants.WIDE + 1L
const val IMPORTED_TEXT = JavaConstants.TEXT + ":" + EXTERNAL
const val IMPORTED_BOOLEAN = JavaConstants.FLAG && TRUE_VALUE
const val IMPORTED_CHAR = "${JavaConstants.SURROGATE}"
const val IMPORTED_FLOAT = JavaConstants.FLOAT * 2
const val IMPORTED_DOUBLE = JavaConstants.DOUBLE * 1.0
const val SOURCE_IMPORT = EXTERNAL + INTERNAL
const val OBJECT_VALUE = Holder.TOTAL + Holder.Nested.NEXT

object Holder {
    private const val SECRET = 5
    const val TOTAL = SECRET + BASE
    @RuntimeConstant(answer = SECRET + 1, text = "secret", flag = true)
    fun marked(): Int = TOTAL
    object Nested {
        const val NEXT = TOTAL + 1
    }
}

@RuntimeConstant(answer = BASE * 2, text = "answer:" + BASE, flag = TRUE_VALUE && JavaConstants.FLAG)
fun annotated(): Int = FORWARD
fun returnedText(): String = TEXT

@RuntimeConstant(answer = ANNOTATION_LATER + 1, text = "forward", flag = true)
fun annotationForward(): Int = 0
const val ANNOTATION_LATER = 9

fun charConcat(value: Char): String = "char" + value

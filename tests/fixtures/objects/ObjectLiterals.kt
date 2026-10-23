package korrust.probes.objects

public object ObjectLiterals {
    const val MIN = -2147483648
    const val MIN_TYPED: Int = -2147483648
    const val MAX: Int = 2147483647
    const val NEGATIVE = -1
    const val ZERO = 0
    const val TRUE = true
    const val FALSE: Boolean = false
    const val NUL = '\u0000'
    const val HIGH: Char = '\uD800'
    const val LOW = '\uDFFF'
    const val LAST: Char = '\uFFFF'
    const val NEWLINE = '\n'
    const val QUOTE: Char = '\''
    const val BACKSLASH = '\\'
    const val EMPTY: String = ""
    const val TEXT = "A\u0000\uD800Z\uDC00\uFFFF漢\uD83D\uDE00"
    const val ESCAPES: String = "\\\t\b\n\r\'\"\$"
}

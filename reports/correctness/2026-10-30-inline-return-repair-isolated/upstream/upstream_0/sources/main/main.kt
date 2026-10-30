fun box(): String {
    if (!isBoolean(true)) return "Fail 1"
    if (isBoolean(0)) return "Fail 2"
    if (!isChar('c')) return "Fail 3"
    if (isChar(0)) return "Fail 4"
    if (!isByte(0.toByte())) return "Fail 5"
    if (isByte(0)) return "Fail 6"
    if (!isShort(0.toShort())) return "Fail 7"
    if (isShort(0)) return "Fail 8"
    if (!isInt(0)) return "Fail 9"
    if (isInt("")) return "Fail 10"
    if (!isLong(0L)) return "Fail 11"
    if (isLong(0.0)) return "Fail 12"
    if (!isFloat(10.0f)) return "Fail 13"
    if (isFloat("")) return "Fail 14"
    if (!isDouble(1.0)) return "Fail 15"
    if (isDouble(0)) return "Fail 16"

    if (!isReifiedInt<Int>()) return "Fail 17"
    if (isReifiedInt<Any>()) return "Fail 18"

    if (1::class != Int::class) return "Fail 19"
    if ('c'::class == ""::class) return "Fail 20"

    return "OK"
}

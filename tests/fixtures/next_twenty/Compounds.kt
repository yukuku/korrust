package next.probes

fun compoundInt(value: Int): Int { var n = value; n -= 2; n *= 3; n /= 2; n %= 7; return n }
fun compoundLong(value: Long): Long { var n = value; n += 2L; n -= 1L; n *= 3L; n /= 2L; n %= 7L; return n }
fun compoundFloat(value: Float): Float { var n = value; n += 2f; n -= 1f; n *= 3f; n /= 2f; n %= 7f; return n }

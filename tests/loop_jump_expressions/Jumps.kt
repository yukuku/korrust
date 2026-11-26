fun count(stop: Int, missing: String?): Int {
    var total = 0
    var i = 0
    outer@ while (i < 6) {
        i += 1
        try {
            val s = if (i == stop) missing ?: break@outer else "abc"
            val n = if (i == 2) missing ?: continue else s
            total += n.length
        } finally {
            total += 10
        }
    }
    return total
}

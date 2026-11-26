package fixtures
fun length(s: String?): Int { consume(s!!.length); return s.length }
fun number(i: Int?): Int { val first = i!! + 1; return first + i }

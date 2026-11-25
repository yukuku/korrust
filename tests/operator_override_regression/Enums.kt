package overrideoperators

enum class Choice(val code:Int=7) {
 FIRST { override fun toString():String="first" },
 SECOND
}
fun enumOrder():Int=Choice.FIRST.compareTo(Choice.SECOND)

import lib.*
fun text(): String = fail("text")
fun number(): Int = fail("number")
fun optional(): String? = absent()
fun standard(): String = error("standard")

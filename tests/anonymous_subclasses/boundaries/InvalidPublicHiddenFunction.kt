interface First { fun first(): Int }
interface Second { fun second(): Int }
private fun hidden() = object : First, Second { override fun first(): Int = 1; override fun second(): Int = 2 }
fun exposed() = hidden()

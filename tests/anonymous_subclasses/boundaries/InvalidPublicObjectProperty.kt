interface First { fun first(): Int }
interface Second { fun second(): Int }
object Holder { val exposed = object : First, Second { override fun first(): Int = 1; override fun second(): Int = 2 } }

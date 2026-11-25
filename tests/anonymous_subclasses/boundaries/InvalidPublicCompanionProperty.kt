interface First { fun first(): Int }
interface Second { fun second(): Int }
class Holder { companion object { val exposed = object : First, Second { override fun first(): Int = 1; override fun second(): Int = 2 } } }

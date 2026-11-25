package anonymousexposure
interface First { fun first(): Int }
interface Second { fun second(): Int }
fun explicit(): First = object : First, Second {
    override fun first(): Int = 1
    override fun second(): Int = 2
}
val explicitValue: First = object : First, Second {
    override fun first(): Int = 3
    override fun second(): Int = 4
}
private fun hidden() = object : First, Second {
    override fun first(): Int = 5
    override fun second(): Int = 6
}
fun readPrivate(): Int = hidden().second()
class Holder {
    val explicitValue: First = object : First, Second {
        override fun first(): Int = 7
        override fun second(): Int = 8
    }
}
object Single {
    val explicitValue: First = object : First, Second {
        override fun first(): Int = 9
        override fun second(): Int = 10
    }
}
class CompanionOwner {
    companion object {
        val explicitValue: First = object : First, Second {
            override fun first(): Int = 11
            override fun second(): Int = 12
        }
    }
}

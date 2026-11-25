class Left
class Right
class Ambiguous {
 operator fun contains(value:Left?):Boolean=true
 operator fun contains(value:Right?):Boolean=false
}
fun invalid():Boolean = null in Ambiguous()

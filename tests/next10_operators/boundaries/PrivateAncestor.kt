open class Parent { private operator fun unaryMinus(): Int = 0 }
class Child: Parent() { fun unaryMinus(): Int = 1 }
fun invalid(): Int = -Child()

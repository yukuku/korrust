open class Parent { open operator fun unaryMinus(): Parent = this }
class Child: Parent() { override fun unaryMinus(): Child = this }
fun bounded(): Child = -Child()

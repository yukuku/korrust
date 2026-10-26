open class Parent { fun value(): Int = 1 }
class Child: Parent() { override fun value(): Int = 2 }

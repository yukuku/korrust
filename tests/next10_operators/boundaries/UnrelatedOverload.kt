open class Parent { operator fun compareTo(other: Parent): Int = 0 }
class Child: Parent() { fun compareTo(other: Int): Int = 0 }
fun invalid(): Boolean = Child() < 3

import factory.*
fun items(): List<Base> = listOf(First(), Second())
fun reverse(): List<Base> = listOf(Second(), First())
fun componentName(): String = component(First(), Second())

package factory
open class Base
class First : Base()
class Second : Base()
fun <T> component(vararg values: T): String = values.javaClass.componentType.name

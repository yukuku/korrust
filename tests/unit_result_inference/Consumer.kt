interface Sink { fun perform() }
class Action : Sink { override fun perform() = Unit }
fun singleton() = Unit
fun explicit(): Unit = Unit
fun asObject(): Any = Unit
class OtherUnit
fun shadowed(Unit: OtherUnit) = Unit

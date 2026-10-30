class Value(var amount: Int)
class Mutation {
    var amount: Int = 99
    fun Value.update(): Int { amount = amount + 1; return amount + this@Mutation.amount }
    fun result(value: Value): Int = value.update()
}
class Tower(val suffix: String) {
    fun String.decorate(): String = this + suffix
    fun nested(other: Tower): String = with(other) { "O".decorate() }
}
open class RootValue
class DerivedValue : RootValue()
open class StaticScope {
    open fun RootValue.pick(): String = "base"
    fun DerivedValue.pick(): String = "derived"
    fun base(value: RootValue): String = value.pick()
    fun derived(value: DerivedValue): String = value.pick()
}
class VirtualScope : StaticScope() { override fun RootValue.pick(): String = "child" }
class Real { fun choose(): String = "member" }
class Priority { fun Real.choose(): String = "extension"; fun result(): String = Real().choose() }
class Generic<T> { fun T.echo(): T = this; fun result(value: T): T = value.echo() }
class GenericProperty<T> { val T.echo: T get() = this; fun result(value: T): T = value.echo }
fun labeled(): Int = with(Value(3)) { this@with.amount }
class Props { val String.amount: Int get() = this@amount.length; fun result(): Int = "OK".amount }
class Effects {
    var events: String = ""
    fun receiver(): String { events += "R"; return "O" }
    fun argument(): String { events += "A"; return "A" }
    fun String.combine(a: String = this, b: String = argument()): String = a + b
    fun result(): String { val value = receiver().combine(a = argument()); return events + value }
}
class CustomMutation {
    var amount: Int get() = 99; set(value) {}
    fun Value.update(): Int { amount = amount + 1; return amount + this@CustomMutation.amount }
    fun result(value: Value): Int = value.update()
}
class MethodGeneric { fun <T> T.echo(): T = this; fun result(): String = "OK".echo() }
open class ParentGeneric<T> { open fun T.render(): T = this; fun run(value: T): T = value.render() }
class ChildGeneric : ParentGeneric<String>() { override fun String.render(): String = this + "K" }

class Host(val suffix: String) { fun String.decorate(): String = this + suffix; fun Host.same(): Boolean = this.suffix == this@Host.suffix; fun String?.sizeOrZero(): Int = this?.length ?: 0; fun result(other: Host): Boolean = other.same() }
open class Parent { open fun String.render(): String = "base" + this; fun run(value: String): String = value.render() }
class Child: Parent() { override fun String.render(): String = "child" + this }

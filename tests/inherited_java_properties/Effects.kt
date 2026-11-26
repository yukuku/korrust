import beans.Base
import beans.GenericBase

class Child : Base() {
    fun rename(value: String): String { this.name = value; return name }
    fun toggle(): Boolean { this.isReady = !isReady; return isReady }
}
class Overriding : Base() {
    override fun getName(): String = "override:" + super.getName()
    override fun setName(value: String) { super.setName(value + "!") }
}
class GetterOverride : Base() {
    override fun getName(): String = "getter:" + super.getName()
}
class KotlinSetter : Base() {
    var recorded: String = ""
    fun setReadOnly(value: String) { recorded = value }
}
class GenericChild : GenericBase<String>()
class Pure { fun getOnly(): String = "pure" }
fun read(value: Child): String = value.name
fun write(value: Child, name: String) { value.name = name }
fun optional(value: Child): String? = value.optional
fun writeOptional(value: Child, text: String?) { value.optional = text }
fun url(value: Child): String = value.url
fun overridden(value: Overriding, text: String): String { value.name = text; return value.name }
fun getterOverride(value: GetterOverride, text: String): String { value.name = text; return value.name }
fun kotlinSetter(value: KotlinSetter, text: String) { value.readOnly = text }
fun generic(value: GenericChild, text: String): String { value.value = text; return value.value }

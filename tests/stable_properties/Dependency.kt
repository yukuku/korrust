package stable.dependency
class Box(val text: String?, val child: Box?)
class MutableBox(var text: String?)
open class OpenBox(open val text: String?)
class CustomBox {
    val text: String? = "custom"
        get() = field
}
class DelegatedBox { val text: String? by lazy { "delegated" } }

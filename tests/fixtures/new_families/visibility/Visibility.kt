package visibilityprobe
import visibilityprobe.Effects
class PrivateDefault {
 var value: Int = 1
  private set
 fun assign(item: Int) { value = item }
 fun deferred(item: Int): () -> Unit = { value = item }
}
class CustomPrivate {
 var value: Int = 1
  private set(item) { field = item + 1 }
 fun assign(item: Int) { value = item }
}
open class ProtectedDefault {
 var value: Int = 1
  protected set
}
class InternalDefault {
 internal var value: Int = 1
}
class PublicInternalSetter {
 var value: Int = 1
  internal set
}
class ExplicitGetter {
 var value: Int = 1
  public get
  private set
}
internal var top: Int = 1
 private set
object State {
 var value: Int = 1
  private set
 internal var hidden: Int = 1
 fun assign(item: Int) { value = item }
}
class Host {
 companion object {
  var value: Int = 1
   private set
  internal var hidden: Int = 1
  fun assign(item: Int) { value = item }
 }
}
class SafeTarget {
 var value: Int = 0
  set(item) { Effects.mark("S"); field = item }
}
fun receiver(value: SafeTarget?): SafeTarget? { Effects.mark("R"); return value }
fun safeAssign(value: SafeTarget?, item: Int) { receiver(value)?.value = Effects.value(item) }
class OtherTarget {
 var value: Int = 0
  private set
 fun assignOther(other: OtherTarget, item: Int) { other.value = item }
 fun deferredOther(other: OtherTarget, item: Int): () -> Unit = { other.value = item }
}
class CustomOther {
 var value: Int = 0
  private set(item) { field = item + 1 }
 fun assignOther(other: CustomOther, item: Int) { other.value = item }
 fun deferredOther(other: CustomOther, item: Int): () -> Unit = { other.value = item }
}
class SetterName {
 var value: Int = 0
  private set
 fun setValue(value: Int) { this.value = value + 1 }
}

interface Contract {
 companion object {
  var value: Int = 1
   private set
  internal var hidden: Int = 1
  fun assign(item: Int) { value = item }
 }
}
class WideTarget {
 var value: Long = 1L
  private set
 fun assignOther(other: WideTarget, item: Long) { other.value = item }
 fun deferredOther(other: WideTarget, item: Long): () -> Unit = { other.value = item }
}
class NullableTarget {
 var value: String? = null
  private set
 fun assignOther(other: NullableTarget, item: String?) { other.value = item }
}
fun at(values: List<String>, index: Int): String? = values.getOrNull(index)
fun mutableAt(values: MutableList<String>, index: Int): String? = values.getOrNull(index)

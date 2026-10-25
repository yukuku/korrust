package declarations.probes
import declarations.support.JavaBase
import declarations.support.Trace
open class Parent(val value: Int) { open fun compute(): Int = value + 1 }
class Child(value: Int): Parent(value) { override fun compute(): Int = super.compute() + 2 }
fun inherited(value: Parent): Int = value.compute()
abstract class AbstractParent { protected val offset = 3; abstract fun abstractCompute(): Int; fun call(): Int = abstractCompute() }
class ConcreteChild: AbstractParent() { override fun abstractCompute(): Int = offset + 4 }
fun abstractInherited(value: AbstractParent): Int = value.call()
class JavaChild(value: Int): JavaBase(value) { override fun compute(): Int = super.compute() + protectedValue() + 2; fun protectedCapture(): Int { val read: () -> Int = { protectedValue() }; return read() } }

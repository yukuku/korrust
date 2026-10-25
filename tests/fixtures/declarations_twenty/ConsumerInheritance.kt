package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val value: Parent = Child(4); check(inherited(value) == 7); check(abstractInherited(ConcreteChild()) == 7); val javaBase: declarations.support.JavaBase = JavaChild(4); check(javaBase.compute() == 12); check(JavaChild(4).protectedCapture() == 5); println("OK") }

package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val value: BodyContract = BodyClass(value = 4); check(value.calculate() == 11); check(value.weighted("abc", 1.5f) == 4.5f); check(BodyClass(3).inferred() == 4); val capture = PrivateCapture(7); check(capture.calc() == 8); check(capture.invokePrivate() == 8); check(BodyObject.weighted("abc", 1.5f) == 4.5f); println("OK") }

package compatibility.consumer
import compatibility.annotations.Annotated
import compatibility.visibility.InternalObject
import compatibility.visibility.InternalClass
import compatibility.visibility.sameModule
import compatibility.visibility.visible
import compatibility.visibility.privateA
import compatibility.visibility.privateB

fun main() {
    val annotated = Annotated(4)
    check(annotated.plus(3) == 7)
    annotated.value = 9
    check(annotated.plus(1) == 10)
    check(sameModule() == 23)
    check(visible(2) == 20)
    check(InternalObject.publicCall(4) == 11)
    check(InternalClass().publicValue(2) == 21)
    check(InternalObject.sameFilePrivate(2) == 24)
    check(privateA() == 6)
    check(privateB() == 10)
    println("consumer-ok")
}

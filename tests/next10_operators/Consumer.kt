package consumer
import operators.Measure
import operators.Shifted
import operators.Plain
import operators.unaryPlus
import operators.unaryMinus
import operators.not
import operators.compareTo
fun main() {
    val source: operators.DerivedFlag = operators.Switch()
    check(!source == 7)
    check(!operators.Switch() == 7)
    check(-operators.Descendant() == 3)
    check((-Shifted(4)).value == 24)
    check(Shifted(4) >= Measure(999))
    check((+Plain(2)).value == 3)
    check((-Plain(2)).value == -2)
    check(!Plain(2) == "custom")
    check(Plain(2) < Plain(3))
    System.out.print("OK")
}

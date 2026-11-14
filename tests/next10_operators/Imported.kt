package imported
import unusual.Dial
import unusual.External
import unusual.Ranking
import unusual.unaryPlus
import unusual.unaryMinus
import unusual.not
import unusual.compareTo
import unusual.makeLeft
import unusual.makeRight
fun members(): Int = (+(-Dial(6))).value
fun notMember(): Long = !Dial(7)
fun extensions(): Int = (+(-External(6))).value
fun notExtension(): String = !External(1)
fun compareMember(a: Int, b: Int, mode: Int): Boolean = when(mode) {
    0 -> Dial(a) < Dial(b)
    1 -> Dial(a) <= Dial(b)
    2 -> Dial(a) > Dial(b)
    else -> Dial(a) >= Dial(b)
}
fun compareExtension(a: Int, b: Int): Boolean = External(a) < External(b)
fun compiledInherited(): Int = (-unusual.Child(4)).value
fun compiledInheritedCompare(): Boolean = unusual.Child(4) >= unusual.Parent(99)
fun interfaceCall(a: Ranking, b: Ranking): Boolean = a < b
fun compiledOrder(): Boolean = makeLeft(1) < makeRight(2)
fun compiledUnaryOrder(): String = !makeLeft(7)
fun compiledCovariant(value: unusual.Down): unusual.Down = -value

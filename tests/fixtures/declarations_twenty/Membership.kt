package declarations.probes
import declarations.support.Trace
import declarations.library.Bag
fun membership(bag: Bag<String>, value: String): Boolean = value in bag
fun absent(bag: Bag<String>, value: String): Boolean = value !in bag
class ConcreteBag { operator fun contains(value: Int): Boolean = value == 7 }
fun concreteMembership(value: Int): Boolean = value in ConcreteBag()
class OrderedBag { operator fun contains(value: Int): Boolean { Trace.mark(3); return value == 1 } }
fun orderedBag(): OrderedBag { Trace.mark(2); return OrderedBag() }
fun containsOrder(): Boolean = Trace.mark(1) in orderedBag()

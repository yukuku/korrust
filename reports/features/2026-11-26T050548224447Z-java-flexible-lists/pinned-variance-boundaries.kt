import variance.Bags
import variance.Box
import variance.GenericBox
fun invalid0(value: MutableList<String>): Int = Bags.accept(value)
fun invalid1(value: MutableList<String>): Int = Bags.choose(value)
fun invalid2(holder: Bags, value: MutableList<String>): Int = holder.select(value)
fun invalid3(holder: Bags, value: MutableList<String>) { holder.setValue(value) }
fun invalid4(value: MutableList<String>) = Box(value)
fun invalid5(value: MutableList<String>) = GenericBox<CharSequence>(value)
fun invalid6(): Int = Bags.accept(Bags.strings())

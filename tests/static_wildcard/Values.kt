package star.consumer
import star.library.First.*
fun integer(): Int = choose(5)
fun text(): String = choose("value")
fun constant(): Int = TOKEN
fun inferred() = choose(7)
fun local(choose: (Int) -> Int): Int = choose(5)
fun captured(): Int { val callback: (Int) -> Int = { choose(it) }; return callback(9) }
fun reference(): Int { val callback: (Int) -> Int = ::choose; return callback(11) }
fun nested(): Int = Nested().answer()
fun field(): Int = state
fun mutate(): Int { state = state + 1; return state }

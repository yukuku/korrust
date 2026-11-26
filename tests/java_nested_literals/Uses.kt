package nestedliteraluses
import nestedliteral.Outer
import nestedliteral.Outer.Nested as Imported

fun nested(): Class<*> = Outer.Nested::class.java
fun imported(): Class<*> = Imported::class.java
fun inner(): Class<*> = Outer.Inner::class.java
fun contract(): Class<*> = Outer.Contract::class.java
fun mode(): Class<*> = Outer.Mode::class.java
fun jdk(): Class<*> = java.util.Map.Entry::class.java

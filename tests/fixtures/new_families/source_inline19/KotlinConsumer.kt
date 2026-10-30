package consumer
import probe.*
import lambdas.*
import forward.*
fun main() {
 check(twice(7)==14)
 check(test<String>("x"))
 check(!test<Int>("x"))
 check(test<String?>(null))
 check(cast<String>("s")=="s")
 check(safe<String>(2)==null)
 check(token<Int>()==Int::class)
 check(physical<Int>()==Int::class.javaObjectType)
 check(relay<String>("x"))
 check(outer(2)==15)
 check(apply(5) { it * 2 }==22)
 check(genericApply(3) { "v" + it }=="v3")
 check(runCross { 31 }==31)
 val function = returnNo { 8 }
 check(function()==8)
 println("checks=14")
}

package compatibility.declarations.consumer
import compatibility.declarations.*
fun main() {
    val mapper = Mapper { value -> value * 3 }
    check(mapper.apply(4) == 12 && mapper.twice(4) == 24)
    check(newMapper(5).apply(8) == 13)
    val captured = capturedMapper(2)
    check(captured.apply(3) == 5 && captured.apply(4) == 9)
    check(captured.twice(1) == 21 && captured.apply(0) == 11)
    check(capturedSink("p") == "p:one;p:two;")
    check(distinguishTypes() == "sam:function")
    check(overloadChoices() == "6:11:16:16:FLSMFT")
    Effects.reset()
    var total = 2
    val direct = dispatch { value -> total += value; total }
    val sam = dispatch(Mapper { value -> total += value; total })
    val function: (Int) -> Int = { value -> total += value; total }
    val typed = dispatch(function)
    check(direct == 7 && sam == 12 && typed == 17 && total == 17)
    check(Effects.events == "FSF")
    println("OK")
}

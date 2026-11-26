package model
fun read(): Int = Outer.get().Hidden(Outer.arg()).n
fun empty(o: Outer): Int = o.Empty().number()
fun nullOuter(): Int = Outer.nil().Hidden(Outer.arg()).n

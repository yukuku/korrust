import capturedvarargs.*
fun consume():String {
 val b=Box<String>("seed")
 val first:()->Int=b.first(7,8)
 val bounded:()->String=b.bounded(Named("bound"))
 val elements:()->String=b.elements(0,"class")
 val size:()->Int=b.size(1,2,3)
 return first().toString()+bounded()+elements()+size().toString()
}

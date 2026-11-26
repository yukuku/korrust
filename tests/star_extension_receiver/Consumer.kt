import store.*
fun read(box: Box<*>): Any? = box.first()
fun copy(box: Box<*>): Box<*> = box.duplicate()
fun copyMap(input: Map<String, *>) = input.toMap()

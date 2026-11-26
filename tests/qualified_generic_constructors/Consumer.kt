fun kotlinBox() = enclosing.Box("value").value
fun javaRef(): String? = java.lang.ref.WeakReference("word").get()
fun explicit(): String = enclosing.Box<String>("explicit").value
fun contextual(): java.util.ArrayList<String> = java.util.ArrayList()

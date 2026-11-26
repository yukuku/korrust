interface Strict { fun accept(values: List<String>): Int }
class Invalid : Strict { override fun accept(values: List<String?>): Int = 0 }

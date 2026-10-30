package annotation17extra
import annotation17.*
import annotation17.Mode.SECOND as PICK
class Bad(val PICK: Int) { @Rich(mode = PICK) fun value(): Int = 1 }

package samconstructors
import samconstructors.Action
import samconstructors.Action as ImportedAction
import samconstructors.Mapper
import java.lang.Runnable
fun interface Local { fun apply(value: Int): Int }
var effects = 0
fun imported(seed: Int): ImportedAction = ImportedAction { value -> seed + value }
fun samePackage(seed: Int): Nearby = Nearby { value -> seed * value }
fun qualified(seed: Int): Action = samconstructors.Action { value -> seed - value }
fun javaNested(seed: Int): External.Operation = External.Operation { value -> seed + value }
fun javaDefault(): Runnable = Runnable { effects += 1 }
fun generic(): Mapper<String> = Mapper<String> { value -> value + "!" }
fun sourceDefined(): Local = Local { value -> value + 5 }
fun capture(): Action {
    var state = 1
    return Action { value -> state += value; state }
}

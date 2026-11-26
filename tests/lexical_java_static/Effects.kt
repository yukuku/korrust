package fieldsuse
import fields.Base
class Child : Base() {
 fun direct(): Int = CODE
 fun captured(): Int { val value: () -> Int = { CODE + counter }; return value() }
 fun sam(): Int { var saved = 0; val action = Runnable { saved = CODE + counter }; action.run(); return saved }
 fun labelValue(): String = label
 fun local(): Int { val CODE = 3; return CODE }
}
class Own : Base() { val CODE: Int = 41; fun own(): Int = CODE }
open class Middle : Base()
class Grand : Middle() { fun inherited(): Int = CODE }
open class GetterBase : Base() { val CODE: Int get() = 43 }
class GetterChild : GetterBase() { fun inheritedGetter(): Int = CODE }
open class StoredBase : Base() { val CODE: Int = 47 }
class StoredChild : StoredBase() { fun inheritedStored(): Int = CODE }

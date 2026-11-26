package markers.consumer
import markers.library.InternalBox
internal fun friend(): InternalBox = InternalBox()
internal fun friendValue(): Long = InternalBox().key.raw

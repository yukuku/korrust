import fieldsuse.Child
import fieldsuse.Own
import fieldsuse.Grand
import importedfield.Imported
import interfacestatic.importedMark
fun consume(): Int { val child = Child(); return child.direct() + child.captured() + Imported().code() + Own().own() + child.local() + Grand().inherited() + importedMark() }

package annotationboundsuses

import android.app.Application
import org.robolectric.annotation.Config

@Config(application = Application::class, sdk = [34], manifest = Config.NONE, shadows = [String::class])
class Robolectric

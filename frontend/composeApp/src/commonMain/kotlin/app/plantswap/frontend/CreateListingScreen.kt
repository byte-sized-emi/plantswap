package app.plantswap.frontend

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.OutlinedTextField
import androidx.compose.material.Switch
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.preat.peekaboo.image.picker.SelectionMode
import com.preat.peekaboo.image.picker.rememberImagePickerLauncher
import com.preat.peekaboo.ui.camera.CameraMode
import com.preat.peekaboo.ui.camera.PeekabooCamera
import com.preat.peekaboo.ui.camera.rememberPeekabooCameraState

@Composable
fun CreateListingScreen(navController: NavController) {
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var tradePossible by remember { mutableStateOf(false) }

    var capturingImage by remember { mutableStateOf(false) }
    var capturedImages: List<ByteArray> by remember { mutableStateOf(mutableListOf()) }

    if (capturingImage) {
        MultiImageCapturer(
            onNewImages = { capturedImages += it },
            capturingImage = capturingImage,
            onCapturingImageChange = { capturingImage = it }
        )
    } else {
        Column(
            modifier = Modifier.fillMaxWidth().padding(30.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {

            Text(
                text = "Create new listing",
                style = MaterialTheme.typography.h4
            )

            Text("Captured ${capturedImages.size} images")

            MultiImageCapturer(
                onNewImages = { capturedImages += it },
                capturingImage = capturingImage,
                onCapturingImageChange = { capturingImage = it }
            )

            OutlinedTextField(
                value = title,
                onValueChange = { title = it },
                label = { Text("Title") },
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = description,
                onValueChange = { description = it },
                label = { Text("Description") },
                minLines = 3,
                singleLine = false,
                modifier = Modifier.fillMaxWidth(),
            )

            Row(verticalAlignment = Alignment.CenterVertically) {
                Switch(
                    checked = tradePossible,
                    onCheckedChange = { tradePossible = it },
                )
                Text("Trade possible")
            }
        }
    }
}

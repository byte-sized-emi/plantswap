package app.plantswap.frontend

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.OutlinedTextField
import androidx.compose.material.Switch
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SnapshotMutationPolicy
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.preat.peekaboo.image.picker.toImageBitmap
import io.ktor.client.HttpClient
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

@OptIn(ExperimentalUuidApi::class)
@Composable
fun CreateListingScreen(navController: NavController, httpClient: HttpClient) {
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var tradePossible by remember { mutableStateOf(false) }
    var listingType by remember { mutableStateOf(ListingType.Buying) }

    var capturingImage by remember { mutableStateOf(false) }
    var imageId by rememberSaveable { mutableStateOf(1) }
    val capturedImages: MutableMap<Int, ImageBitmap> by rememberSaveable { mutableStateOf(mutableMapOf()) }

    val addImage: (ByteArray) -> Unit = { image ->
        capturedImages[imageId] = image.toImageBitmap()
        imageId++
    }

    val handleCreateListing = {
        val listing = Listing(
            title = title,
            description = description,
            tradeable = tradePossible,
            listingType = listingType
        )
        // TODO: Ktor request to backend to upload all images
        // TODO: Ktor request to create listing
        // TODO: Switch navigation to a view of the newly created listing
    }

    if (capturingImage) {
        MultiImageCapturer(
            onNewImages = { it.forEach { image -> addImage(image) } },
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

            Row(modifier = Modifier.height(50.dp).fillMaxWidth()) {
                capturedImages.forEach { image ->
//                    Box {
//                        IconButton(onClick = { capturedImages.remove(image.key) }) {
//                            Icon(Icons.Filled.Delete, contentDescription = "Delete this image")
//                        }
                        Image(bitmap = image.value, contentDescription = "captured image preview")
//                    }
                }
            }

            MultiImageCapturer(
                onNewImages = { it.forEach { image -> addImage(image) } },
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
                    checked = tradeable,
                    onCheckedChange = { tradeable = it },
                )
                Text("Trade possible")
            }

            Row(verticalAlignment = Alignment.CenterVertically) {
                Switch(
                    checked = listingType == ListingType.Buying,
                    onCheckedChange = {
                        listingType = if (listingType == ListingType.Buying) {
                            ListingType.Selling
                        } else {
                            ListingType.Buying
                        }
                    },
                )
                Text("Buying?")
            }

            Button(onClick = { handleCreateListing() }) {
                Text("Create Listing")
            }
        }
    }
}

<template>
  <div>
    <v-card-text
      class="mt-2"
      style="text-align: center; padding: 0">

      <span v-if="!isOnly">Unknown (depends on input)</span>
      <span v-else-if="shape.length == 0">{{ content[0] }}</span>
      <span v-else-if="isSmall" style="white-space: pre;">{{ matrix.toString() }}</span>
      <div v-else>
        <v-btn
          large color="primary"
          @click="displayDialog = true">
          Display
        </v-btn>
        <v-btn
          large
          @click="download">
          Export
        </v-btn>
      </div>
    </v-card-text>
    <v-dialog v-model="displayDialog" max-width="850px">
      <v-card>
        <v-card-title>
          <span>Edge value:</span>
        </v-card-title>
        <v-card-text
          style="text-align: center; white-space: pre;">{{ matrix.toString() }}</v-card-text>
      </v-card>
    </v-dialog>
  </div>
</template>

<script>
  import numjs from 'numjs'

  numjs.config.printThreshold = 7;

  export default {
    props: ['value'],

    data: () => ({
      displayDialog: false
    }),

    computed: {
      isOnly() {
        return !!this.value.Only
      },

      isSmall() {
        return this.shape.every(d => d <= numjs.config.printThreshold)
      },

      shape() {
        return this.value.Only[1]
      },

      content() {
        return this.value.Only[2]
      },

      matrix() {
        return numjs.array(this.content).reshape(this.shape)
      },
    },

    methods: {
      download() {
        let data = this.matrix.toJSON()

        let blob = new Blob([data], {type: 'text/json'})
        let e = document.createEvent('MouseEvents')
        let a = document.createElement('a')

        a.download = 'edge-content.json'
        a.href = window.URL.createObjectURL(blob)
        a.dataset.downloadurl =  ['text/json', a.download, a.href].join(':')
        e.initMouseEvent('click', true, false, window, 0, 0, 0, 0, 0, false, false, false, false, 0, null)
        a.dispatchEvent(e)
      }
    }
  }
</script>

import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type ImageFile = {
  name: string;
  content: string | ArrayBuffer | null;
  type: string;
};

const initialState: {
  images: ImageFile[];
} = {
  images: [],
};

export const attachedImagesSlice = createSlice({
  name: "attachedImages",
  initialState: initialState,
  reducers: {
    addImage: (state, action: PayloadAction<ImageFile>) => {
      state.images = state.images.concat(action.payload);
    },
    removeImageByIndex: (state, action: PayloadAction<number>) => {
      state.images = state.images.filter(
        (_image, index) => index !== action.payload,
      );
    },
    resetAttachedImagesSlice: () => {
      return initialState;
    },
  },
  selectors: {
    selectAllImages: (state) => state.images,
  },
});

export const { selectAllImages } = attachedImagesSlice.selectors;
export const { addImage, removeImageByIndex, resetAttachedImagesSlice } =
  attachedImagesSlice.actions;

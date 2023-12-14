import React from 'react'
import { Box } from '@radix-ui/themes'
import styles from './sidebar.module.css'

export const Sidebar: React.FC<React.PropsWithChildren> = (props) => {
    return (<Box {...props} className={styles.sidebar} />)
}